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
use pixel_physics::sim::plant;
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
    /// **The phenotype beside the genome census, split drifted / undrifted.**
    ///
    /// `CLAUDE.md`, and this line's own handoff §6: *a genome that changed is
    /// not a plant that changed* — a mistake made three times here already.
    /// Every column above counts *tables*, so a rate sweep read off them alone
    /// says how much the population's bookkeeping moved and nothing about what
    /// grew. These four are the far side of the call.
    ///
    /// `mature` is the fraction of an individual's body cells that reached
    /// `MatureBody`, and it is the specific readout for what an emptied `Grow`
    /// slot does: `self_type_after_grow` falls back to the cell's own type, so
    /// the tip never *retires* — the lineage advances as a frontier of tips
    /// that never thickens and never anchors. Deformed, not dead, so a
    /// cell count alone cannot see it and an establishment count barely can.
    /// Seeds are excluded from the denominator: a plant is not less mature for
    /// carrying seeds, and herb carries a great many.
    cells_drifted: f32,
    cells_base: f32,
    mature_drifted: f32,
    mature_base: f32,
    /// Denominators for the four above, printed because a mean over two
    /// individuals is not a mean.
    n_drifted: usize,
    n_base: usize,
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
    // **The two knobs that are read from the environment, echoed for the
    // same reason every other parameter above is.** These are worse than the
    // argv ones, not better: an argv typo at least appears in the shell
    // history, while `PIXEL_PHYSICS_FATE_LOOKUP=genom` (one letter short)
    // silently runs the default and produces a log that is indistinguishable
    // from a correct one. A log that does not name its mode was written by a
    // binary that never had one.
    println!(
        "genome_drift: fate_lookup={:?} fate_mutation_chance={} (defaults: GenomeOnly, {})",
        plant::fate_lookup_mode(),
        plant::fate_mutation_chance_seed(),
        plant::FATE_MUTATION_CHANCE
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

    // **Distinct individuals, not organism-samples.** The first version of
    // this pooled `fate_established` across samples and divided -- which
    // counts the same plant once per sample it survived, so a single drifted
    // plant that persists reads as nine, and a run where none established
    // reads as a rate over ~1,000 trials rather than as "none, ever". Two
    // seeds came back at *exactly* 0.00% and that tidiness was the tell: at a
    // 1% rate, P(0 of 989) is about 4e-5. `CLAUDE.md`'s "ask what your number
    // counts" -- the percentage was right and the denominator was a different
    // quantity from the one the question needs.
    let mut ever_established: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut ever_established_drifted: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    // **The denominators the two sets above do not have**, and without which
    // `ed / e` cannot be read as a viability rate at all: it is the share of
    // *establishers* that carried drift, and the null it has to beat is the
    // share of *everyone* that carried drift. Holding only the numerators
    // makes a 2x2 look like a rate. Both are distinct-individual sets on the
    // same footing as the two above, so the four divide cleanly into
    // establishment rate within the drifted and within the rest.
    let mut ever_seen: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut ever_drifted: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();

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
        // (body cells, of which MatureBody) per individual, for the phenotype
        // columns. Read off the grid rather than off `OrganismState::cells`
        // because `OrganismCell` carries resources and support distance and
        // *not* the cell type -- the type lives in the grid cell's `aux`.
        let mut body: std::collections::BTreeMap<u16, (u32, u32)> = std::collections::BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                let c = w.get(x, y);
                if c.organism_id() == 0 {
                    continue;
                }
                owners.insert(c.organism_id());
                match organism::cell_type(c.aux()) {
                    Some(organism::CellType::Seed) => {
                        seed_owners.insert(c.organism_id());
                    }
                    Some(t) => {
                        let e = body.entry(c.organism_id()).or_insert((0, 0));
                        e.0 += 1;
                        if t == organism::CellType::MatureBody {
                            e.1 += 1;
                        }
                    }
                    None => {}
                }
            }
        }

        let mut n = 0.0f32;
        let (mut fd, mut fl, mut fs, mut fc, mut f0, mut fe) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        let mut fate_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut sum = [0.0f64; organism::GENOTYPE_TRAITS];
        let mut sumsq = [0.0f64; organism::GENOTYPE_TRAITS];
        let (mut gen_sum, mut gen_max) = (0u64, 0u16);
        // The phenotype accumulators, split by whether the individual's
        // production rule is still its species'. Only individuals with a
        // *body* enter them: a bare seed has 0 body cells, so a mature
        // fraction is undefined for it and pooling it in would report the
        // seed bank as immature plants.
        let (mut pn_d, mut pn_b) = (0usize, 0usize);
        let (mut pc_d, mut pc_b) = (0.0f64, 0.0f64);
        let (mut pm_d, mut pm_b) = (0.0f64, 0.0f64);
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
            // `>= 20 cells` is `plant_probe`'s and `fate_viability`'s
            // establishment bar, reused deliberately so the three instruments
            // are answering about the same object.
            let established = s.cells.len() >= 20;
            ever_seen.insert(*id);
            if established {
                ever_established.insert(*id);
            }
            let g = s.fates;
            if g.is_empty() {
                fe += 1;
            } else {
                fate_keys.insert(g.rules().map(|(o, r)| format!("{o:?}/{r:?};")).collect::<String>());
                let drifted = g != base_fates;
                if drifted {
                    ever_drifted.insert(*id);
                    fd += 1;
                    if s.generation == 0 {
                        f0 += 1;
                    }
                    if established {
                        ever_established_drifted.insert(*id);
                    }
                    match g.len().cmp(&base_fates.len()) {
                        std::cmp::Ordering::Greater => fl += 1,
                        std::cmp::Ordering::Less => fs += 1,
                        std::cmp::Ordering::Equal => fc += 1,
                    }
                }
                if let Some((cells, mature)) = body.get(id).copied().filter(|(c, _)| *c > 0) {
                    let (cells, mature) = (cells as f64, mature as f64);
                    if drifted {
                        pn_d += 1;
                        pc_d += cells;
                        pm_d += mature / cells;
                    } else {
                        pn_b += 1;
                        pc_b += cells;
                        pm_b += mature / cells;
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
            cells_drifted: if pn_d > 0 { (pc_d / pn_d as f64) as f32 } else { f32::NAN },
            cells_base: if pn_b > 0 { (pc_b / pn_b as f64) as f32 } else { f32::NAN },
            mature_drifted: if pn_d > 0 { (pm_d / pn_d as f64) as f32 } else { f32::NAN },
            mature_base: if pn_b > 0 { (pm_b / pn_b as f64) as f32 } else { f32::NAN },
            n_drifted: pn_d,
            n_base: pn_b,
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
    // **`empty` is the second control.** An empty genome reports as *no
    // drift* while meaning *no genome* -- indistinguishable from a lineage
    // that simply never mutated, which is the failure this project has shipped
    // three times as "a channel with no writer". What it means downstream
    // changed on 2026-08-30 and this comment said the old thing: under the
    // shipped `FateLookup::GenomeOnly` an empty genome no longer falls back to
    // the species table, it answers `None` to every query, so a nonzero
    // `empty` is now a plant with no production rule at all rather than one
    // quietly running its species'. `FateGenome::mutate`'s delete floor is
    // what should make it unreachable; this column is where that floor
    // failing would show.
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

    // **The phenotype beside the genome census.** Every column above counts
    // tables. This one asks whether the plants those tables built are
    // different plants, which is a separate question and the one a rate sweep
    // actually turns on -- `Reports/lanes/plant-evolution-handoff-2026-08-30.md`
    // §6: *a genome that changed is not a plant that changed*, three arms
    // vacuous before they were real.
    //
    // `mature` is the fraction of body cells that reached `MatureBody`, and it
    // is aimed at one specific deformation: an emptied `Grow` slot leaves
    // `self_type_after_grow` falling back to the cell's own type, so a tip
    // never retires and the lineage runs on as a frontier of tips -- it never
    // thickens and never anchors. That plant is deformed rather than dead, so
    // it still has cells and can still establish; only the mature fraction
    // separates it from a healthy one.
    println!("\nphenotype, drifted against undrifted (body cells only; seeds excluded):");
    println!(
        "  {:>8} {:>7} {:>9} {:>7} {:>9} {:>7} {:>9}",
        "frame", "n drift", "cells", "mature", "n base", "cells", "mature"
    );
    for s in &samples {
        println!(
            "  {:>8} {:>7} {:>9.1} {:>7.3} {:>9} {:>9.1} {:>7.3}",
            s.frame, s.n_drifted, s.cells_drifted, s.mature_drifted, s.n_base, s.cells_base, s.mature_base
        );
    }
    println!("  (NaN means the column had no individuals at that sample, not a value of zero.)");
    // **Does a drifted rule table cost a plant its establishment?** The
    // harness gate (`fate_viability`) asks this of one mutation at a time in a
    // constructed stand; this asks it of a living population, which is a
    // different and weaker question -- it cannot separate "the mutation is bad"
    // from "this individual was unlucky" -- but it is the only in-vivo form
    // available, and the two rates being equal is a real null.
    let last_d = last.fate_drifted;
    let last_n = last.live;
    let e = ever_established.len();
    let ed = ever_established_drifted.len();
    println!("\ndoes a drifted table cost establishment?");
    let pc = |a: usize, b: usize| if b == 0 { f32::NAN } else { 100.0 * a as f32 / b as f32 };
    println!("  drifted among all organisms, final sample   {last_d} of {last_n}  ({:.2}%)", pc(last_d, last_n));
    println!("  DISTINCT plants that ever established       {e}");
    println!("  ...of which ever carried a drifted table    {ed}  ({:.2}%)", pc(ed, e));
    println!("  (distinct individuals, counted once each -- a plant that persists across samples");
    println!("   is one plant. The per-sample columns above are snapshots and do repeat it.)");
    println!("  Equal rates are the null. A lower established rate is viability selection against");
    println!("  the drifted; a higher one would mean drift is reaching establishment preferentially,");
    println!("  which nothing in the model provides for and would point at this census instead.");
    // The 2x2 the two lines above are the numerators of -- the line above is a
    // *composition* (what share of establishers drifted) and moves with the
    // mutation rate whether or not viability changes at all.
    //
    // **And this pair is confounded by age, which was measured rather than
    // reasoned about, so do not read it as viability either.** Drift
    // accumulates down a lineage, so a drifted individual is on average
    // *younger* than an undrifted one -- the undrifted set holds the founders
    // and every early birth -- and a younger individual has had less time to
    // reach the 20-cell bar. Measured 2026-08-30 on `herb`, seed 1, 20,000
    // frames at rate 0.10: this pair reads **1.89% against 5.75%**, an
    // apparent 3x penalty, in a run whose stand is **bit-identical** to the
    // same seed at rate 0 -- same live count, same establishers, same
    // germinations, same slot means at every sample. Drift there provably
    // changed nothing, so a 3x cost cannot be a cost.
    //
    // What *is* a viability instrument is the paired same-seed comparison
    // against rate 0: run the identical world with the rate at zero and read
    // whether the establisher count, the body sizes and the throughput move.
    // That cancels age, assignment and competition together.
    let seen = ever_seen.len();
    let dr = ever_drifted.len();
    let base_seen = seen.saturating_sub(dr);
    let base_est = e.saturating_sub(ed);
    println!("  establishment rate WITHIN each group -- CONFOUNDED BY AGE, not a viability rate:");
    println!("    drifted     {ed} established of {dr} ever seen   ({:.2}%)", pc(ed, dr));
    println!("    undrifted   {base_est} established of {base_seen} ever seen   ({:.2}%)", pc(base_est, base_seen));
    println!("    (drift accumulates down a lineage, so the drifted are younger and have had less");
    println!("     time to reach 20 cells. Measured: this pair reads 1.89% vs 5.75% on a herb run");
    println!("     whose stand is bit-identical to the same seed at rate 0. For viability, run the");
    println!("     same seed at rate 0 and compare the establisher count and body sizes.)");

    // **Throughput, cumulative over the whole run.** Everything above is a
    // standing count at one instant; a rate that quietly shuts the
    // reproductive engine down would show there only as a smaller `n`, which
    // is also what a crowded stand looks like. These two are monotone, so
    // they cannot be confused with a snapshot.
    println!("\nthroughput over the whole run:");
    println!("  germinations                 {}", w.germinations);
    println!("  fruit/seeds dropped          {}", w.fruit_dropped);

    // **Where the drift went: the draw counted at its source.**
    //
    // `Reports/plant-rule-drift-observed-2026-08-29.md` §4 left a 2.6x gap
    // between the drift a standing census sees and the drift a per-birth
    // model predicts, and could not attribute it: a census reads genomes,
    // never the draws that made them. These three counters come from
    // `plant::bear_seed_at` itself, so the gap splits into segments that
    // each fail differently.
    //
    // **Read the first row first.** The draw is *not* a per-birth roll: it
    // comes from a substream keyed on `(world seed, landing cell, parent
    // generation)`, so every seed landing on one cell from same-generation
    // parents gets the same answer, and the realised rate over births is a
    // ratio whose denominator is how births pile onto keys. It has far more
    // spread than a binomial at the same n, and a single run sitting well
    // off the nominal rate is the expected behaviour of that design rather
    // than evidence of a bug.
    let rolls = w.fate_mutation_rolls;
    let fired = w.fate_mutations_fired;
    let applied = w.fate_mutations_applied;
    let pcu = |a: u64, b: u64| if b == 0 { f32::NAN } else { 100.0 * a as f32 / b as f32 };
    println!("\nfate mutations, counted at the source (`plant::bear_seed_at`):");
    println!("  births that reached the draw   {rolls}");
    println!(
        "  ...where the draw fired        {fired}  ({:.3}% of births, nominal {:.3}%)",
        pcu(fired, rolls),
        100.0 * plant::fate_mutation_chance_seed()
    );
    println!(
        "  ...that changed the genome     {applied}  ({:.1}% of draws applied, rest declined)",
        pcu(applied, fired)
    );
    // The effect counter on the far side. `applied` is cumulative over the
    // whole run and `fate_drifted` is a standing count at one instant, so
    // these are not two measurements of one quantity -- the gap between
    // them is everything that removes a mutated genome from the world
    // (death, a seed that never establishes) plus everything that hides one
    // (a later mutation landing back on the species value).
    println!(
        "  standing drifted genomes       {last_d}  ({:.1}% of the {applied} ever applied)",
        pcu(last_d as u64, applied)
    );
    // The model the 2.6x was measured against, recomputed here from this
    // run's own numbers so the two never drift apart in a report.
    let predicted = 1.0 - (1.0 - plant::fate_mutation_chance_seed()).powf(last.gen_mean);
    println!(
        "  per-birth model at mean generation {:.2}: {:.2}% of the population, against {:.2}% observed",
        last.gen_mean,
        100.0 * predicted,
        pc(last_d, last_n)
    );
    println!("  (the model assumes one independent draw per birth in a lineage's ancestry, which");
    println!("   the keyed substream above does not provide. Compare the FIRST row to the nominal");
    println!("   rate before reading anything into the last one.)");

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
