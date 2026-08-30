//! **Is there a standing pile of food beside the nest?** The pre-flight for
//! `store_in_body`, the granary-versus-replete gene.
//!
//! `Reports/creature-reproduction-economics.md` §5.3 proposes a heritable
//! `store_in_body`: an animal's surplus sits either in a granary beside the
//! nest or in its own body. The owner's ruling (2026-08-30) is that the two
//! forks must be **reachable outcomes, not two hand-built species** — so
//! before the gene is written, both ends of its codomain have to be shown
//! to exist.
//!
//! **This exists because a gene with one reachable end expresses nothing,
//! and this project has already paid for that once.** Plants'
//! `light_weight` was authored up to 0.6 while `phototropism_dir` could
//! only return up-or-nothing; fixing the codomain took reproduction to zero
//! because every constant had been calibrated against the broken quantity
//! (`CLAUDE.md`, *Fixing a bug often exposes a constant that was
//! compensating for it*). The replete end needs no world state at all — it
//! is a number on an organism. The granary end needs cells in the world,
//! and *that* is the half that can fail to exist.
//!
//! # What is actually claimed, and what is measured
//!
//! `creature.rs`'s drop branch writes the carried cell into the world when
//! the ant is adjacent to a nest cell ("At the nest it is storage and
//! always wanted"), and increments `deliveries`. The predation pre-flight
//! logged 532 of them. **Nothing in the engine spends from the resulting
//! pile.** So the question is not whether the verb fires — it demonstrably
//! does — but whether the verb leaves anything standing.
//!
//! Four questions, and each has its own column:
//!
//! * does a pile **persist**, or does it rot, get buried, get eaten?
//! * how **large** does it get, and does it plateau?
//! * is it ever **eaten**, and by whom?
//! * over what **horizon**, and over how many seeds?
//!
//! # The three things this file is careful about
//!
//! 1. **A census of every food cell in the world is not a census of the
//!    pile.** That is verbatim one of the six recorded instances of this
//!    repo's worst-recurring failure — *a census counted every `Solid` in
//!    the world rather than the platform under test*. So the census is
//!    banded by Chebyshev distance to the nearest nest cell (multi-source
//!    BFS, computed once), the world-wide figure is printed **beside** the
//!    banded one rather than instead of it, and `mode=control` proves the
//!    two differ by planting a pile far from the nest and checking the
//!    banded census does *not* see it.
//! 2. **Both halves of the control, not just the positive one.** The
//!    specificity half asks what the number reads when nothing is wrong (a
//!    world with no colony still has litter near the nest, because leaves
//!    fall); the sensitivity half asks whether it moves when something is
//!    (a hand-planted pile must read as one). Five of six wrong numbers in
//!    the recorded session needed the second and had only the first.
//! 3. **Persistence is measured without depending on delivery.** A planted
//!    pile in a colony-free world answers "does a pile survive here" on its
//!    own terms; if ants never accumulate one, that arm still says whether
//!    a granary *could* stand. The 2x2 (`ants` x `planted`) is the whole
//!    design: the ants-on-planted-pile arm is the one that says whether
//!    anything eats it.
//!
//! Every arm shares its world seed and its frame indices with the other
//! three, so the day/night and water cycles are common-mode and an
//! arm-to-arm difference read at one frame is not a phase difference
//! (`CLAUDE.md`, *a designed oscillator must be divided out*). The absolute
//! trajectory is printed as a trajectory for the same reason: any single
//! reading off it carries that frame's phase.
//!
//! ```text
//! cargo run --release --example larder_probe -- mode=control
//! cargo run --release --example larder_probe -- seeds=18 frames=6000
//! cargo run --release --example larder_probe -- seeds=1 frames=12000 every=400
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::{creature, parallel, Cell, World};

/// The scene `predation_probe` and `creature_space::run_one` build — a
/// nest strip cut into a wetland surface, two trees for a renewable food
/// source, and a colony walked out from the nest. **Reproduced rather than
/// invented**: the 532 deliveries this probe is about were logged on this
/// world, and a census taken on a different world than the claim it is
/// checking checks nothing.
const W: i32 = 512;
const H: i32 = 160;
const ANTS: usize = 52;
const TREES: i32 = 2;
const PRESET: &str = "wetland";
const WARMUP: usize = 2400;
/// Columns the nest strip occupies.
const NEST_X0: i32 = 16;
const NEST_X1: i32 = 90;

/// Bands, in Chebyshev cells from the nearest nest cell. **2 is the tight
/// one and it is not a guess**: `act` drops into an empty 8-neighbour of a
/// head that is itself an 8-neighbour of a nest cell, so every delivery
/// lands at distance <= 2 by construction. The wider bands exist because a
/// powder pile spreads and a granary that has slumped four cells downhill
/// is still a granary.
const BANDS: [i32; 4] = [2, 4, 8, 16];
const BAND_CAP: i32 = 16;

const SEED_BASE: u64 = pixel_physics::sim::world::DEFAULT_WORLD_SEED;

fn main() {
    let mut frames = 6000usize;
    let mut every = 500usize;
    let mut seeds = 18u64;
    let mut plant = 40usize;
    let mut mode = "census".to_string();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "seeds" => seeds = v.parse().expect("seeds"),
            "plant" => plant = v.parse().expect("plant"),
            "mode" => mode = v.to_string(),
            // **An unknown argument is silently ignored, and that has cost
            // this repo a 3.5-hour study** (`CLAUDE.md`). Panic instead.
            other => panic!("unknown arg {other:?}; known: mode, frames, every, seeds, plant"),
        }
    }

    // **Echo the parameters.** A log that does not name its own seed was
    // written by a binary that never had one.
    println!(
        "larder_probe: mode={mode} frames={frames} every={every} seeds={seeds} plant={plant} \
         scene={PRESET} {W}x{H} ants={ANTS} trees={TREES} warmup={WARMUP} nest_x={NEST_X0}..{NEST_X1} bands={BANDS:?}\n"
    );

    match mode.as_str() {
        "census" => census(frames, every, seeds, plant),
        "control" => control(plant),
        "render" => render(frames, every),
        "pair" => pair(frames),
        "turnover" => turnover(frames, every),
        other => panic!("unknown mode {other:?}; known: census, control, render, pair, turnover"),
    }
}

// ---------------------------------------------------------------------------
// what a run reports
// ---------------------------------------------------------------------------

/// One census of the world at one frame.
#[derive(Default, Clone)]
struct Sample {
    frame: usize,
    /// **Free food, by band.** Free means `organism_id() == 0` — carrion
    /// and dropped mouthfuls belong to nobody, living tissue belongs to
    /// somebody (`creature.rs`'s `is_living_kin` says so, and it is the
    /// same test). A living leaf on a tree is standing crop, not a
    /// granary, and counting it as one is the whole failure this split
    /// exists to avoid.
    free_cells: [u32; BANDS.len()],
    /// Summed `creature::food_value` over those cells — **the continuous
    /// quantity**, and the one a birth would actually be paid from.
    /// `CLAUDE.md` prefers a summed deficit over a count of bad cells for
    /// exactly this reason: a count gives knife-edge margins.
    free_worth: [f64; BANDS.len()],
    /// The same cells put through `diet_yield` at the **ant's own gut**.
    /// `food_value` is what a mouthful is worth to anybody; S5's matched
    /// filter decides what this animal gets out of swallowing it, and the
    /// two differ by the gut. Quoting face value as "what the larder is
    /// worth to the colony" is the same class of error as quoting the
    /// world-wide count as the larder: arithmetically correct, about a
    /// different question.
    free_yield: [f64; BANDS.len()],
    /// Organism-owned edible cells in the widest band — standing crop, the
    /// alternative explanation for any food seen near the nest.
    owned_cells: u32,
    /// **The wrong number, printed on purpose.** Free edible cells over the
    /// whole world. It is here so the banded figure can be read against it;
    /// quoting this one *as* the larder is the recorded failure.
    world_free_cells: u32,
    /// Free edible cells in the tight band, split by material.
    by_material: Vec<(String, u32)>,
    ants_alive: usize,
    deliveries: u64,
    pickups: u64,
    drops: u64,
    eats: u64,
    deaths: u64,
}

/// Everything one arm-run reports, reduced.
#[derive(Default, Clone)]
struct Row {
    seed: u64,
    arm: &'static str,
    samples: Vec<Sample>,
}

impl Row {
    fn last(&self) -> Sample {
        self.samples.last().cloned().unwrap_or_default()
    }
    /// Largest tight-band pile ever observed. **Read against the last
    /// sample**: a max far above the final says the pile forms and is then
    /// consumed, which is a different world from one where it never forms.
    fn peak_tight(&self) -> u32 {
        self.samples.iter().map(|s| s.free_cells[0]).max().unwrap_or(0)
    }
    fn peak_wide(&self) -> u32 {
        self.samples.iter().map(|s| s.free_cells[BANDS.len() - 1]).max().unwrap_or(0)
    }
}

/// What to put in the world before the run, and where.
#[derive(Clone, Copy, PartialEq)]
enum Plant {
    /// Nothing — the colony builds whatever pile there is, or does not.
    None,
    /// A hand-built pile on the nest strip. The **sensitivity** control,
    /// and independently the persistence experiment: it answers "can a
    /// granary stand here" without depending on ants ever making one.
    AtNest(usize),
    /// The same pile, at the far end of the world. The **specificity**
    /// control: the banded census must not see it while the world-wide
    /// count must.
    FarAway(usize),
}

// ---------------------------------------------------------------------------
// the census
// ---------------------------------------------------------------------------

fn census(frames: usize, every: usize, seeds: u64, plant: usize) {
    println!(
        "free = edible cells owned by no organism (carrion and dropped mouthfuls); owned = edible cells that are\n\
         living tissue (standing crop). <=N is Chebyshev distance to the nearest nest cell; world is the whole map,\n\
         printed beside the banded figure and never instead of it. worth = summed creature::food_value.\n"
    );
    let arms: [(&'static str, bool, Plant); 4] = [
        ("colony", true, Plant::None),
        ("no ants", false, Plant::None),
        ("planted, no ants", false, Plant::AtNest(plant)),
        ("planted + colony", true, Plant::AtNest(plant)),
    ];

    let mut all: Vec<Row> = Vec::new();
    for s in 0..seeds {
        let seed = SEED_BASE + s;
        for &(arm, ants, paint) in &arms {
            let row = run(seed, frames, every, ants, paint, arm);
            all.push(row);
        }
    }

    // --- the trajectory, on the first seed only ------------------------
    // Printed as a trajectory because any single reading off it carries
    // that frame's phase of the day/night and water cycles.
    println!("--- trajectory, seed {} (one seed: the shape, not the answer) ---", SEED_BASE);
    println!("{:>18} {:>7} {:>6} {:>6} {:>6} {:>6} {:>9} {:>9} {:>7} {:>7} {:>7} {:>7} {:>6}",
        "arm", "frame", "<=2", "<=4", "<=8", "<=16", "worth<=2", "yield<=2", "owned", "world", "deliv", "eats", "ants");
    for row in all.iter().filter(|r| r.seed == SEED_BASE) {
        for s in &row.samples {
            println!("{:>18} {:>7} {:>6} {:>6} {:>6} {:>6} {:>9.0} {:>9.0} {:>7} {:>7} {:>7} {:>7} {:>6}",
                row.arm, s.frame, s.free_cells[0], s.free_cells[1], s.free_cells[2], s.free_cells[3],
                s.free_worth[0], s.free_yield[0], s.owned_cells, s.world_free_cells, s.deliveries, s.eats, s.ants_alive);
        }
        println!();
    }

    // --- the order statistic, over every seed --------------------------
    // Six seeds is not a sweep (measured: 1.64x over six, 1.08x over the
    // next twelve, pooled median zero), so the headline is a distribution
    // and never a mean.
    // **The label is `frames`, not a multiple of `every`.** `run` pushes a
    // final sample at `frames` on top of the cadence, so `last()` is that
    // sample; the first version of this line computed the last *cadence*
    // frame and printed 15000 over a table read at 18000. A table headed
    // with a frame it was not taken at is the same defect as a metric
    // measuring the wrong quantity, wearing a smaller hat.
    println!("--- settled pile at frame {frames}, over {seeds} seed(s): order statistics ---");
    println!("{:>18} {:>26} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "arm", "quantity", "min", "p10", "med", "p90", "max", "peak med", "n>0");
    for &(arm, _, _) in &arms {
        let rows: Vec<&Row> = all.iter().filter(|r| r.arm == arm).collect();
        // **The world-wide row is in the same table on purpose.** It is the
        // number a careless census would have quoted as the larder, and
        // reading it beside the banded rows is what makes the banded ones
        // mean anything -- quoting that contrast off one seed would have
        // been the same single-sample weakness this table exists to avoid.
        for (label, pick) in [
            ("free cells <=2 of nest", 0usize),
            ("free cells <=8 of nest", 2usize),
            ("free cells WORLD-WIDE (not the larder)", 9usize),
        ] {
            let mut v: Vec<f64> = rows.iter().map(|r| if pick == 9 { r.last().world_free_cells as f64 } else { r.last().free_cells[pick] as f64 }).collect();
            let peaks: Vec<f64> = rows
                .iter()
                .map(|r| match pick {
                    0 => r.peak_tight() as f64,
                    9 => r.samples.iter().map(|s| s.world_free_cells).max().unwrap_or(0) as f64,
                    _ => r.peak_wide() as f64,
                })
                .collect();
            let nonzero = v.iter().filter(|&&x| x > 0.0).count();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |f: f64, s: &[f64]| s[((s.len() as f64 - 1.0) * f).round() as usize];
            let mut p = peaks.clone();
            p.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!("{arm:>18} {label:>26} {:>6.0} {:>6.0} {:>6.0} {:>6.0} {:>6.0} {:>8.0} {:>8}",
                q(0.0, &v), q(0.1, &v), q(0.5, &v), q(0.9, &v), q(1.0, &v), q(0.5, &p), nonzero);
        }
    }

    // --- the churn decomposition ---------------------------------------
    // **`deliveries` is the near side of the verb and the pile is the far
    // side**, and they are not the same number. `act` checks the food
    // branch before the drop branch and gates it on `carrying.is_none()`,
    // so a sated ant standing beside its own colony's pile picks a cell
    // *up* — and, still at the nest, puts it back down on a later tick,
    // scoring a second delivery. Deliveries far above the standing pile
    // are that loop, not import.
    println!("\n--- what `deliveries` bought: summed over {} seed(s) ---", seeds);
    println!("{:>18} {:>10} {:>10} {:>9} {:>8} {:>8} {:>14} {:>14}",
        "arm", "deliveries", "pickups", "drops", "eats", "deaths", "peak <=2 (sum)", "final <=2 (sum)");
    for &(arm, _, _) in &arms {
        let rows: Vec<&Row> = all.iter().filter(|r| r.arm == arm).collect();
        let sum = |f: &dyn Fn(&Row) -> u64| rows.iter().map(|r| f(r)).sum::<u64>();
        println!("{arm:>18} {:>10} {:>10} {:>9} {:>8} {:>8} {:>14} {:>14}",
            sum(&|r| r.last().deliveries), sum(&|r| r.last().pickups), sum(&|r| r.last().drops),
            sum(&|r| r.last().eats), sum(&|r| r.last().deaths),
            sum(&|r| r.peak_tight() as u64), sum(&|r| r.last().free_cells[0] as u64));
    }

    // --- what the pile is made of --------------------------------------
    println!("\n--- material of the free cells within 2 of the nest, last sample, summed over seeds ---");
    for &(arm, _, _) in &arms {
        let mut tally: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for r in all.iter().filter(|r| r.arm == arm) {
            for (m, n) in r.last().by_material {
                *tally.entry(m).or_default() += n;
            }
        }
        let list: Vec<String> = tally.iter().map(|(m, n)| format!("{m} {n}")).collect();
        println!("{arm:>18}  {}", if list.is_empty() { "(nothing)".to_string() } else { list.join(", ") });
    }

    // --- the paired difference the arms exist for ----------------------
    // **Paired means per-seed, and this line got it wrong once.** The first
    // version differenced the two arms' *medians*, printed it under the
    // heading "paired, per-seed", and was neither: a difference of medians
    // discards the pairing that the shared seed exists to provide, and on
    // a distribution this wide (colony <=2 ranges 0 to 47 across seeds) the
    // two statistics need not even agree in sign. The seed is what cancels
    // the terrain, so the difference has to be taken inside the seed and
    // the order statistic taken over those differences.
    let per_seed = |a: &str, b: &str, pick: usize| -> Vec<f64> {
        (0..seeds)
            .filter_map(|s| {
                let seed = SEED_BASE + s;
                let ra = all.iter().find(|r| r.arm == a && r.seed == seed)?;
                let rb = all.iter().find(|r| r.arm == b && r.seed == seed)?;
                Some(ra.last().free_cells[pick] as f64 - rb.last().free_cells[pick] as f64)
            })
            .collect()
    };
    let stat = |mut v: Vec<f64>| -> (f64, f64, f64, usize, usize) {
        v.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let q = |f: f64, s: &[f64]| s[((s.len() as f64 - 1.0) * f).round() as usize];
        let up = v.iter().filter(|&&d| d > 0.0).count();
        let down = v.iter().filter(|&&d| d < 0.0).count();
        (q(0.1, &v), q(0.5, &v), q(0.9, &v), up, down)
    };
    println!(
        "\ntruly paired: the difference taken WITHIN each seed, then the order statistic over those {seeds} differences.\n\
         (same seed, same frame, so terrain, the water cycle and the day cycle all cancel.)"
    );
    println!("{:>44} {:>7} {:>7} {:>7} {:>18}", "comparison", "p10", "med", "p90", "seeds up/down");
    for (label, a, b, pick) in [
        ("colony - no ants, free cells <=2 of nest", "colony", "no ants", 0usize),
        ("colony - no ants, free cells <=8 of nest", "colony", "no ants", 2),
        ("planted+colony - planted-no-ants, <=2", "planted + colony", "planted, no ants", 0),
    ] {
        let (p10, med, p90, up, down) = stat(per_seed(a, b, pick));
        println!("{label:>44} {p10:>+7.0} {med:>+7.0} {p90:>+7.0} {:>18}", format!("{up} up / {down} down"));
    }

    // The per-seed rows behind those differences, printed so the paired
    // statistic never has to be re-derived by re-running an hour of sweep --
    // which is exactly what fixing this line cost the first time.
    println!("\nper-seed, free cells <=2 of nest at frame {frames}:");
    println!("{:>6} {:>9} {:>9} {:>7} {:>18} {:>18}", "seed", "colony", "no ants", "delta", "planted+colony", "planted, no ants");
    for s in 0..seeds {
        let seed = SEED_BASE + s;
        let at = |arm: &str| all.iter().find(|r| r.arm == arm && r.seed == seed).map_or(0, |r| r.last().free_cells[0]);
        println!("{:>6} {:>9} {:>9} {:>+7} {:>18} {:>18}",
            s, at("colony"), at("no ants"), at("colony") as i64 - at("no ants") as i64, at("planted + colony"), at("planted, no ants"));
    }

    // --- and the only conversion that answers the brief ----------------
    // **The question is not "are there cells", it is "could a birth be paid
    // from them".** So the stock is priced in the currency the decision is
    // taken in, at the same `birth_cost` the engine charges, and reported
    // at the *peak* as well as at the end: a granary that reaches one child
    // and is then eaten is a reachable granary; one that never reaches a
    // fraction of a child is not.
    let cost = ant_birth_cost();
    let peak_yield = |arm: &str| -> f64 {
        all.iter()
            .filter(|r| r.arm == arm)
            .map(|r| r.samples.iter().map(|s| s.free_yield[0]).fold(0.0f64, f64::max))
            .sum::<f64>()
            / all.iter().filter(|r| r.arm == arm).count().max(1) as f64
    };
    let last_yield = |arm: &str| -> f64 {
        let rows: Vec<&Row> = all.iter().filter(|r| r.arm == arm).collect();
        rows.iter().map(|r| r.last().free_yield[0]).sum::<f64>() / rows.len().max(1) as f64
    };
    println!(
        "\npriced against creature::birth_cost = {cost:.0} (the engine's own charge, not an estimate):\n  \
         {:>18}  peak larder <=2 = {:>8.0} digestible = {:>5.2} births   |   final = {:>8.0} = {:>5.2} births\n  \
         {:>18}  peak larder <=2 = {:>8.0} digestible = {:>5.2} births   |   final = {:>8.0} = {:>5.2} births",
        "colony", peak_yield("colony"), peak_yield("colony") / cost as f64, last_yield("colony"), last_yield("colony") / cost as f64,
        "no ants", peak_yield("no ants"), peak_yield("no ants") / cost as f64, last_yield("no ants"), last_yield("no ants") / cost as f64,
    );
}

/// What the engine charges for a birth, read off the shipped species rather
/// than copied out of a report — a constant transcribed into a harness is a
/// constant that silently stops matching the code.
fn ant_birth_cost() -> f32 {
    let world = World::new(Rect::new(0, 0, 7, 7));
    let id = world.species.id_of("ant").expect("ant species");
    let def = world.species.get(id).creature.as_ref().expect("ant is a creature");
    creature::birth_cost(def)
}

// ---------------------------------------------------------------------------
// the controls
// ---------------------------------------------------------------------------

/// **Both halves, because they are different checks and conflating them is
/// how five of six wrong numbers got through** (`CLAUDE.md`).
///
/// *Specificity* — what does this read when nothing is wrong? A pile at the
/// far end of the world must not register as a larder, while the world-wide
/// column must see it. That is the recorded failure (a census counting every
/// cell of a material world-wide) tested directly rather than avoided by
/// intention.
///
/// *Sensitivity* — does it move when something is? A pile planted on the
/// nest strip must register, at close to the planted count.
fn control(plant: usize) {
    let plant = plant.max(20);
    println!("controls: a hand-planted pile of {plant} cells, at the nest and at the far end of the world.\n");

    let bare = run(SEED_BASE, 600, 200, false, Plant::None, "bare");
    let near = run(SEED_BASE, 600, 200, false, Plant::AtNest(plant), "planted at nest");
    let far = run(SEED_BASE, 600, 200, false, Plant::FarAway(plant), "planted far away");

    println!("{:>18} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10}", "arm", "<=2", "<=4", "<=8", "<=16", "world", "worth<=4", "yield<=4");
    for r in [&bare, &near, &far] {
        let s = r.samples.first().cloned().unwrap_or_default();
        println!("{:>18} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10.0} {:>10.0}",
            r.arm, s.free_cells[0], s.free_cells[1], s.free_cells[2], s.free_cells[3], s.world_free_cells, s.free_worth[1], s.free_yield[1]);
    }

    let b = bare.samples[0].clone();
    let n = near.samples[0].clone();
    let f = far.samples[0].clone();

    // Sensitivity.
    assert!(
        n.free_cells[1] >= b.free_cells[1] + (plant as u32) / 2,
        "a pile of {plant} planted on the nest moved the <=4 band by only {} — the census is blind, not quiet",
        n.free_cells[1] as i64 - b.free_cells[1] as i64
    );
    assert!(n.free_worth[1] > b.free_worth[1], "the worth column did not move for a planted pile");
    // Specificity.
    assert!(
        f.world_free_cells >= b.world_free_cells + (plant as u32) / 2,
        "the world-wide column did not see a pile of {plant} planted in it — the far arm never placed anything, so the specificity check is vacuous"
    );
    assert!(
        f.free_cells[3] <= b.free_cells[3] + 2,
        "a pile planted {}+ cells from the nest registered in the <=16 band ({} against a bare {}): the census is counting the world, not the larder",
        W - NEST_X1, f.free_cells[3], b.free_cells[3]
    );
    println!(
        "\n  PASS sensitivity: a planted pile moves the banded census (<=4: {} -> {}).\
         \n  PASS specificity: the same pile at the far end moves the world column ({} -> {}) and not the banded one (<=16: {} -> {}).",
        b.free_cells[1], n.free_cells[1], b.world_free_cells, f.world_free_cells, b.free_cells[3], f.free_cells[3]
    );

    // --- and the persistence half, which is a measurement, not a check --
    println!("\npersistence of a planted {plant}-cell pile at the nest, with and without a colony:");
    let alone = run(SEED_BASE, 6000, 1000, false, Plant::AtNest(plant), "planted, no ants");
    let with_ants = run(SEED_BASE, 6000, 1000, true, Plant::AtNest(plant), "planted + colony");
    println!("{:>18} {:>7} {:>7} {:>7} {:>8} {:>8}", "arm", "frame", "<=2", "<=8", "world", "eats");
    for r in [&alone, &with_ants] {
        for s in &r.samples {
            println!("{:>18} {:>7} {:>7} {:>7} {:>8} {:>8}", r.arm, s.frame, s.free_cells[0], s.free_cells[2], s.world_free_cells, s.eats);
        }
    }
}

// ---------------------------------------------------------------------------
// is it a store, or is it a flow?
// ---------------------------------------------------------------------------

/// **A standing count of ten cannot tell a granary of ten from ten cells
/// permanently in transit**, and those are different answers to the gene
/// question: the first is a store `store_in_body` could trade against, the
/// second is a queue. So the tight band is tracked as a *set* of occupied
/// positions and the entries and exits between samples are counted.
///
/// The pairing is deliberate (`CLAUDE.md`: pair every "it fired" counter
/// with an effect counter from the far side of the call). `deliveries` is
/// the near side — the verb ran. `entries` is the far side — a cell that
/// was not in the larder now is. `exits` says what happened to it after.
fn turnover(frames: usize, every: usize) {
    let mut world = build_scene(SEED_BASE, true, Plant::None);
    let dist = nest_distance(&world);
    let bias = world
        .species
        .id_of("ant")
        .and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.traits[TRAIT_GUT_BIAS]))
        .expect("ant species");

    let occupied = |w: &World| -> std::collections::HashSet<(i32, i32)> {
        let mut set = std::collections::HashSet::new();
        for y in 0..H {
            for x in 0..W {
                if dist[(y * W + x) as usize] > BANDS[0] {
                    continue;
                }
                let c = w.get(x, y);
                if c.material != material::EMPTY && c.organism_id() == 0 && creature::diet_yield(w, c, bias) > creature::EAT_YIELD_THRESHOLD {
                    set.insert((x, y));
                }
            }
        }
        set
    };

    println!("the tight band (<=2 of a nest cell) as a set, sampled every {every} frames.\n\
              entries/exits are cumulative; `resident` is the cells present at BOTH this sample and the first nonempty one.\n");
    println!("{:>7} {:>7} {:>9} {:>8} {:>9} {:>10} {:>9}", "frame", "cells", "entries", "exits", "resident", "deliveries", "eats");

    let mut prev = occupied(&world);
    let mut first: Option<std::collections::HashSet<(i32, i32)>> = None;
    let (mut entries, mut exits) = (0u64, 0u64);
    for f in 0..=frames {
        if f > 0 && f % every == 0 {
            let now = occupied(&world);
            entries += now.difference(&prev).count() as u64;
            exits += prev.difference(&now).count() as u64;
            if first.is_none() && !now.is_empty() {
                first = Some(now.clone());
            }
            let resident = first.as_ref().map_or(0, |f0| f0.intersection(&now).count());
            println!("{:>7} {:>7} {:>9} {:>8} {:>9} {:>10} {:>9}",
                f, now.len(), entries, exits, resident, world.creature_stats.deliveries, world.creature_stats.eats);
            prev = now;
        }
        if f == frames {
            break;
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    println!(
        "\n{entries} entries and {exits} exits against {} deliveries: a delivery that stays is one entry, and a cell\n\
         picked back up and put down again is an exit and an entry. Read the standing count against both.",
        world.creature_stats.deliveries
    );
}

// ---------------------------------------------------------------------------
// the picture
// ---------------------------------------------------------------------------

/// A contact sheet of the nest over time, **from the same world the census
/// ran on**.
///
/// This is a mode here rather than a scene on `filmstrip` for one reason
/// and it is not ownership: `filmstrip`'s `scene=colony` builds a
/// *different* nest (`found_colony`'s 53-column patch, placed by score)
/// than the one the 532-delivery claim and this census were measured on,
/// and a picture of a different world than the claim it illustrates
/// illustrates nothing. The counts printed under each tile come from the
/// identical `sample` call the census uses, so the number beside the image
/// cannot disagree with the number in the table.
fn render(frames: usize, every: usize) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::particle::ParticleSystem;
    use std::collections::HashSet;

    // Crop to the nest and the air above it, zoomed, because a 512x160
    // sheet puts the whole question in a strip 74 px wide and a pile of ten
    // cells is not judgeable at that size.
    const ZOOM: i32 = 4;
    const CROP_W: i32 = 110;
    const CROP_H: i32 = 46;
    let crop_x = NEST_X0 - 4;

    let mut world = build_scene(SEED_BASE, true, Plant::None);
    let dist = nest_distance(&world);
    let bias = world
        .species
        .id_of("ant")
        .and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.traits[TRAIT_GUT_BIAS]))
        .expect("ant species");
    let crop_y = (0..W).map(|x| surface(&world, x)).skip(NEST_X0 as usize).take(40).min().unwrap_or(60) - CROP_H + 12;

    let mut renderer = Renderer::new();
    // **Pin the daylight.** Sampling every 3,000 frames walks straight
    // across the day/night cycle, so half the tiles came back at night and
    // the sheet compared a lit nest with an unlit one -- the oscillator
    // aliasing into the picture exactly as it aliases into a number.
    renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    let particles = ParticleSystem::new();
    let mut frame = vec![0u8; (W * H * 4) as usize];
    let mut tiles: Vec<Vec<u8>> = Vec::new();
    let mut captions: Vec<String> = Vec::new();

    for f in 0..=frames {
        if f % every == 0 {
            renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (W as u32, H as u32), true);
            let mut tile = vec![0u8; (CROP_W * ZOOM * CROP_H * ZOOM * 4) as usize];
            for ty in 0..CROP_H * ZOOM {
                for tx in 0..CROP_W * ZOOM {
                    let (sx, sy) = (crop_x + tx / ZOOM, crop_y + ty / ZOOM);
                    let src = (((sy.clamp(0, H - 1)) * W + sx.clamp(0, W - 1)) * 4) as usize;
                    let dst = ((ty * CROP_W * ZOOM + tx) * 4) as usize;
                    tile[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
            tiles.push(tile);
            let s = sample(&world, &dist, bias, f);
            captions.push(format!("frame {f}: larder <=2 = {} cells ({:.0} digestible), <=8 = {}, deliveries {}", s.free_cells[0], s.free_yield[0], s.free_cells[2], s.deliveries));
        }
        if f == frames {
            break;
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }

    // Two columns, gutters between.
    const GAP: i32 = 6;
    let cols = 2i32;
    let rows = (tiles.len() as i32 + cols - 1) / cols;
    let (tw, th) = (CROP_W * ZOOM, CROP_H * ZOOM);
    let (sw, sh) = (cols * tw + (cols - 1) * GAP, rows * th + (rows - 1) * GAP);
    let mut sheet = vec![24u8; (sw * sh * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }
    for (i, tile) in tiles.iter().enumerate() {
        let (cx, cy) = (i as i32 % cols, i as i32 / cols);
        let (ox, oy) = (cx * (tw + GAP), cy * (th + GAP));
        for y in 0..th {
            let dst = (((oy + y) * sw + ox) * 4) as usize;
            let src = ((y * tw) * 4) as usize;
            sheet[dst..dst + (tw * 4) as usize].copy_from_slice(&tile[src..src + (tw * 4) as usize]);
        }
    }
    let out = "larder_nest.png";
    image::save_buffer(out, &sheet, sw as u32, sh as u32, image::ColorType::Rgba8).expect("write sheet");
    println!("wrote {out} ({sw}x{sh}), crop={crop_x},{crop_y},{CROP_W},{CROP_H} zoom={ZOOM}, tiles left-to-right then down:");
    for c in &captions {
        println!("  {c}");
    }
}

/// **The paired panel, and the only honest way to ask this by eye.** One
/// world has taken every delivery a 52-ant colony makes over `frames`; the
/// other has no colony at all. Everything else about them — seed, terrain,
/// trees, frame, daylight — is identical. If the granary end of
/// `store_in_body` is a thing that exists in the world, the two pictures
/// differ at the nest; a viewer who cannot tell them apart is the finding.
fn pair(frames: usize) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::particle::ParticleSystem;
    use std::collections::HashSet;

    const ZOOM: i32 = 6;
    const CROP_W: i32 = 96;
    const CROP_H: i32 = 34;
    const GAP: i32 = 8;
    let crop_x = NEST_X0 - 2;

    let mut panels: Vec<(Vec<u8>, String)> = Vec::new();
    let mut crop_y = 0;
    for (label, ants) in [("colony", true), ("no colony", false)] {
        let mut world = build_scene(SEED_BASE, ants, Plant::None);
        let dist = nest_distance(&world);
        let bias = world
            .species
            .id_of("ant")
            .and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.traits[TRAIT_GUT_BIAS]))
            .expect("ant species");
        if crop_y == 0 {
            crop_y = (NEST_X0..NEST_X0 + 40).map(|x| surface(&world, x)).min().unwrap_or(60) - CROP_H + 10;
        }
        for _ in 0..frames {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
        }
        let mut renderer = Renderer::new();
        renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
        let particles = ParticleSystem::new();
        let mut frame = vec![0u8; (W * H * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (W as u32, H as u32), true);
        let mut tile = vec![0u8; (CROP_W * ZOOM * CROP_H * ZOOM * 4) as usize];
        for ty in 0..CROP_H * ZOOM {
            for tx in 0..CROP_W * ZOOM {
                let (sx, sy) = ((crop_x + tx / ZOOM).clamp(0, W - 1), (crop_y + ty / ZOOM).clamp(0, H - 1));
                let src = ((sy * W + sx) * 4) as usize;
                let dst = ((ty * CROP_W * ZOOM + tx) * 4) as usize;
                tile[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
            }
        }
        let s = sample(&world, &dist, bias, frames);
        panels.push((
            tile,
            format!("{label}: deliveries {}, larder <=2 of nest {} cells ({:.0} digestible), <=8 {} cells, free food world-wide {}",
                s.deliveries, s.free_cells[0], s.free_yield[0], s.free_cells[2], s.world_free_cells),
        ));
    }

    let (tw, th) = (CROP_W * ZOOM, CROP_H * ZOOM);
    let (sw, sh) = (tw, 2 * th + GAP);
    let mut sheet = vec![24u8; (sw * sh * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }
    for (i, (tile, _)) in panels.iter().enumerate() {
        let oy = i as i32 * (th + GAP);
        for y in 0..th {
            let dst = (((oy + y) * sw) * 4) as usize;
            let src = ((y * tw) * 4) as usize;
            sheet[dst..dst + (tw * 4) as usize].copy_from_slice(&tile[src..src + (tw * 4) as usize]);
        }
    }
    for (i, (tile, _)) in panels.iter().enumerate() {
        image::save_buffer(format!("larder_pair_{i}.png"), tile, tw as u32, th as u32, image::ColorType::Rgba8).expect("write panel");
    }
    image::save_buffer("larder_pair.png", &sheet, sw as u32, sh as u32, image::ColorType::Rgba8).expect("write pair");
    println!("wrote larder_pair.png ({sw}x{sh}) and larder_pair_0/1.png ({tw}x{th} each), crop={crop_x},{crop_y},{CROP_W},{CROP_H} zoom={ZOOM}, top then bottom:");
    for (_, c) in &panels {
        println!("  {c}");
    }
}

// ---------------------------------------------------------------------------
// the scene and the run
// ---------------------------------------------------------------------------

fn build_scene(seed: u64, ants: bool, paint: Plant) -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    world.seed = seed;

    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let params = presets.get(PRESET).expect("wetland preset");
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });

    let surface_at: Vec<i32> = (0..W).map(|x| surface(&world, x)).collect();

    let nest = world.materials.id_of("nest").expect("nest");
    for x in NEST_X0..NEST_X1 {
        world.set(x, surface_at[x as usize], Cell::new(nest, 0).with_attached(true));
    }
    for i in 0..TREES {
        let x = 150 + i * (300 / TREES.max(1));
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    for _ in 0..WARMUP {
        world.step_active_sites();
        world.step_fields();
    }

    if ants {
        // Place until there are ANTS of them, not at ANTS fixed columns:
        // `plant_creature_seed` refuses a site it does not fit and returns
        // quietly (`creature_space`'s own note).
        let mut planted = 0usize;
        let mut x = 24i32;
        while planted < ANTS && x < W - 8 {
            if let Some(site) = creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "ant") {
                world.schedule_active_site(site);
                planted += 1;
                x += 4;
            } else {
                x += 1;
            }
        }
        assert!(planted > 0, "no ant was placed; the scene does not contain the situation this probe is about");
    }

    // The planted pile goes in **after** the warmup, so it is not buried by
    // 2,400 frames of settling before anything has looked at it.
    match paint {
        Plant::None => {}
        Plant::AtNest(n) => plant_pile(&mut world, NEST_X0 + 4, n),
        Plant::FarAway(n) => plant_pile(&mut world, W - 60, n),
    }
    world
}

/// Half litter, half leaf — **the two forms a delivered mouthful can take,
/// and they behave differently**, so a single-material pile would answer
/// half the persistence question. `litter` is a `Powder` that falls and
/// rots into soil on a moisture-gated schedule; `leaf` is `Plant`-kind,
/// which never moves and has no `decays_into` at all.
fn plant_pile(world: &mut World, x0: i32, n: usize) {
    let litter = world.materials.id_of("litter").expect("litter");
    let leaf = world.materials.id_of("leaf").expect("leaf");
    let mut placed = 0usize;
    let mut x = x0;
    while placed < n && x < W - 4 {
        let top = surface(world, x);
        // One cell of each per column, in the two empty cells above the
        // ground, so the pile is one course deep and cannot be dismissed
        // as an unstable tower.
        for (dy, m) in [(1i32, litter), (2, leaf)] {
            if placed >= n {
                break;
            }
            let (px, py) = (x, top - dy);
            if world.get(px, py).material == material::EMPTY {
                world.set(px, py, Cell::new(m, 128));
                placed += 1;
            }
        }
        x += 1;
    }
    assert_eq!(placed, n, "the pile could not be placed; the scene does not contain the situation the control is about");
}

/// Chebyshev distance from every cell to the nearest nest cell, capped at
/// `BAND_CAP`. Multi-source 8-neighbour BFS, run once per scene — the nest
/// is `Solid` and `attached` and ants dig soil rather than nest, so it does
/// not move during a run.
fn nest_distance(world: &World) -> Vec<i32> {
    let nest = world.materials.id_of("nest").expect("nest");
    let idx = |x: i32, y: i32| (y * W + x) as usize;
    let mut dist = vec![i32::MAX; (W * H) as usize];
    let mut frontier: Vec<(i32, i32)> = Vec::new();
    for y in 0..H {
        for x in 0..W {
            if world.get(x, y).material == nest {
                dist[idx(x, y)] = 0;
                frontier.push((x, y));
            }
        }
    }
    assert!(!frontier.is_empty(), "no nest cell in the world; every band would be empty by construction");
    let mut d = 0;
    while !frontier.is_empty() && d < BAND_CAP {
        d += 1;
        let mut next = Vec::new();
        for (x, y) in frontier.drain(..) {
            for (dx, dy) in creature::DIRS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= W || ny >= H {
                    continue;
                }
                if dist[idx(nx, ny)] == i32::MAX {
                    dist[idx(nx, ny)] = d;
                    next.push((nx, ny));
                }
            }
        }
        frontier = next;
    }
    dist
}

fn run(seed: u64, frames: usize, every: usize, ants: bool, paint: Plant, arm: &'static str) -> Row {
    let mut world = build_scene(seed, ants, paint);
    let dist = nest_distance(&world);
    // **The gut the census reads with is the one the ants have**, not a
    // neutral one: `diet_yield` is a matched filter against `food_class`,
    // so "is this edible" is a question about a particular animal. Read off
    // a live ant where there is one, and off the species otherwise, so the
    // no-ants arm asks the identical question.
    let bias = world
        .species
        .id_of("ant")
        .and_then(|id| world.species.get(id).creature.as_ref().map(|d| d.traits[TRAIT_GUT_BIAS]))
        .expect("ant species");

    // **Early samples, on top of the regular cadence.** A census taken
    // long after an event measures the system's *response* rather than the
    // event (`CLAUDE.md`: the same quantity read 369 / 42,825 / 67,100 at
    // 5 / 50 / 1,300 frames after one blast). A pile has the same
    // structure: "how big does the colony's larder get" and "how big is it
    // once the colony has started eating it" are different questions and
    // the onset is where the first is answered.
    const EARLY: [usize; 6] = [50, 100, 200, 400, 800, 1600];

    let mut row = Row { seed, arm, samples: Vec::new() };
    for frame in 0..frames {
        if frame % every == 0 || EARLY.contains(&frame) {
            row.samples.push(sample(&world, &dist, bias, frame));
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    row.samples.push(sample(&world, &dist, bias, frames));
    row
}

fn sample(world: &World, dist: &[i32], bias: f32, frame: usize) -> Sample {
    let mut s = Sample { frame, ..Default::default() };
    let mut mats: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for y in 0..H {
        for x in 0..W {
            let c = world.get(x, y);
            if c.material == material::EMPTY {
                continue;
            }
            // The engine's own predicate, not a material whitelist: what
            // counts as food is `diet_yield` against the threshold `act`
            // uses, so a picture of the pile cannot disagree with what an
            // animal would take from it.
            if creature::diet_yield(world, c, bias) <= creature::EAT_YIELD_THRESHOLD {
                continue;
            }
            let free = c.organism_id() == 0;
            if free {
                s.world_free_cells += 1;
            }
            let d = dist[(y * W + x) as usize];
            if d > BAND_CAP {
                continue;
            }
            if !free {
                if d <= BANDS[BANDS.len() - 1] {
                    s.owned_cells += 1;
                }
                continue;
            }
            let worth = creature::food_value(world, c) as f64;
            let yielded = creature::diet_yield(world, c, bias) as f64;
            for (i, &band) in BANDS.iter().enumerate() {
                if d <= band {
                    s.free_cells[i] += 1;
                    s.free_worth[i] += worth;
                    s.free_yield[i] += yielded;
                }
            }
            if d <= BANDS[0] {
                *mats.entry(c.material.0).or_default() += 1;
            }
        }
    }
    s.by_material = mats.into_iter().map(|(m, n)| (world.materials.get(MaterialId(m)).name.clone(), n)).collect();
    s.ants_alive = world.creature_stats.spawned.saturating_sub(world.creature_stats.deaths) as usize;
    s.deliveries = world.creature_stats.deliveries;
    s.pickups = world.creature_stats.pickups;
    s.drops = world.creature_stats.drops;
    s.eats = world.creature_stats.eats;
    s.deaths = world.creature_stats.deaths;
    s
}

fn surface(world: &World, x: i32) -> i32 {
    (0..H)
        .find(|&y| {
            world.get(x, y).organism_id() == 0
                && matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)
        })
        .unwrap_or(H - 1)
}
