//! **Does one environmental difference produce two different-shaped plants?**
//!
//! Item A4 of `Reports/plant-project-review-2026-08-23.md`, and the
//! measurement `Reports/physical-trees-design-2026-08-23.md` §11.6 is waiting
//! on: *"same founders, windy patch against sheltered patch, scored on
//! root:shoot and on slenderness"*. The owner's end of it is a world where
//! biomes that rarely storm grow thinner-rooted, more slender trees — and
//! nobody can build that until there is an instrument that can tell two
//! patches apart without fooling itself.
//!
//! **This is deliberately not a wind instrument.** `weather::at(seed, frame)`
//! takes no position, so wind is one value for the entire world (§11.5) and
//! there is nothing spatial to vary yet; a separate package is building
//! terrain-derived exposure. So the axis is a parameter and moisture is the
//! setting that exists today. Pointing this at wind when exposure lands is
//! adding one arm to [`Axis`] and nothing else — see that type's doc.
//!
//! ```text
//! cargo run --release --example divergence                       # 8 seeds, scouting length
//! cargo run --release --example divergence -- control=1          # the identical-patch control
//! cargo run --release --example divergence -- frames=25200 seeds=8
//! cargo run --release --example divergence -- species=grass founders=16
//! ```
//!
//! # The three things this is shaped around, each a failure already paid for
//!
//! **1. Two worlds, not two halves of one world.** "Same founders" has to be
//! literal or the comparison is measuring genotype draw as well as
//! environment: genotypes come from `(world seed, germination coordinate)`,
//! so two patches at different x in one world are founded by different
//! plants. Two separately-built worlds at the same seed, the same geometry
//! and the same seed coordinates are founded by *the same* individuals, and
//! the only difference between the runs is the axis. It also makes the
//! control below exact rather than approximate.
//!
//! **2. The control comes first.** `control=1` sets both patches to the same
//! value and the answer has to be a clean zero. A metric that finds a
//! difference between two identical patches is measuring its own noise, and
//! this project has shipped exactly that: the whisker hunt defined a film as
//! "a water cell with air above and below", which is what falling water looks
//! like, so it counted every droplet in the world. Its numbers were real and
//! meant nothing. `CLAUDE.md`: *sanity-check a new metric against a case you
//! know is fine, before trusting it about a case you don't.*
//!
//! **3. An order statistic over seeds, and the spread, never a difference of
//! means.** Twelve identical trees from one genome span 31 to 153 cells. A
//! difference of two means over that spread is not a result, so every figure
//! here is reported with its quartiles beside it, the per-seed divergences
//! are printed individually, and the headline is **how many seeds moved the
//! same way** — which is the statistic that survives a distribution this
//! wide.
//!
//! **Rebuild before trusting a run** (`cargo build --release --examples`):
//! `cargo build --release` alone does *not* rebuild examples, and species
//! files are `include_str!`d, so an edited `.ron` and a prebuilt harness
//! produce a bit-identical "run" that swept nothing.

mod common;

use pixel_physics::sim::material;
use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;
use std::collections::BTreeMap;

/// **What differs between the two patches.**
///
/// One axis at a time, by construction: the instrument's whole claim is that
/// the only difference between the two runs is this, so a second simultaneous
/// axis would make every number here uninterpretable in exactly the way
/// `NICHE_SHARPNESS`'s first sweep was (two knobs moving one metric).
///
/// **Moisture is the only arm today and that is a fact about the engine, not
/// a scope decision.** `weather::at(seed, frame)` takes no position, so wind
/// is global; exposure derived from terrain at gust time
/// (`Reports/physical-trees-design-2026-08-23.md` §11.5) is what would make
/// a windy patch and a sheltered patch expressible at all, and it is another
/// package's work. When it lands, an `Exposure` arm here needs to do one
/// thing: set the patch's exposure the way [`Axis::Moisture`] sets its soil
/// water. Everything downstream — the founders, the control, the metrics,
/// the seed sweep — is already axis-agnostic and does not change.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Starting soil water, on `SOIL_SATURATED`'s scale. The wet arm is field
    /// capacity; the dry arm sits well above the wilting point, because a
    /// patch below it germinates nothing and the comparison would be a
    /// living stand against an empty field rather than two morphologies.
    Moisture,
}

impl Axis {
    fn name(self) -> &'static str {
        match self {
            Axis::Moisture => "moisture",
        }
    }
}

/// One individual's shape, which is the only thing this instrument scores on.
///
/// Both numbers are *ratios*, deliberately. A raw cell count is dominated by
/// how big the plant got, and how big it got is the noisiest quantity in this
/// engine — the same genome spans 31 to 153 cells. A ratio divides that out
/// and leaves the shape, which is what the question is about.
struct Morph {
    /// Root cells over shoot cells. Root is `reinforces_powder` tissue or a
    /// `RootTip`, which is the same test `examples/plant_probe.rs` uses; two
    /// definitions of "root" drifting apart across two harnesses is the
    /// failure `examples/common` exists to end.
    root_shoot: f32,
    /// Height above the anchor plate over stem width at the base.
    ///
    /// **What "the anchor plate" is here, stated rather than implied.**
    /// `plant::is_structural_anchor` is the engine's own answer and it is
    /// private to `plant.rs` — a file two other packages are live in, which
    /// this one is under instruction to stay out of. So the plate is read
    /// off the plant instead: the **topmost row holding root tissue**, which
    /// for a rooted plant is the collar, where the root system ends and the
    /// shoot begins. That is the row the plant is held at, which is what the
    /// design report's phrase means. The day `anchor_support` exposes the
    /// anchor set it already enumerates, this should read that instead —
    /// the numbers should barely move, and if they do, that is worth knowing.
    ///
    /// **Stem width is the count of shoot cells in the lowest shoot row.**
    /// That is the quantity `thicken`/`pipe_ratio` moves at the trunk base,
    /// which is what makes slenderness a reading of the economy rather than
    /// of the walk.
    slenderness: f32,
}

/// Every established individual's shape.
///
/// `MIN_CELLS` is the same establishment floor `plant_probe` uses. Without
/// one, a two-cell seedling that never got going contributes a slenderness
/// of 1/1 and drags a median that is supposed to describe grown plants.
const MIN_CELLS: usize = 20;

fn measure(w: &World) -> Vec<Morph> {
    let Some(b) = w.bounds() else { return Vec::new() };
    // Per organism: root cells, shoot cells, topmost root row, topmost shoot
    // row, lowest shoot row, and the shoot cells in each row (for the width).
    let mut root: BTreeMap<u16, usize> = BTreeMap::new();
    let mut shoot: BTreeMap<u16, usize> = BTreeMap::new();
    let mut plate_top: BTreeMap<u16, i32> = BTreeMap::new();
    let mut shoot_top: BTreeMap<u16, i32> = BTreeMap::new();
    let mut shoot_base: BTreeMap<u16, i32> = BTreeMap::new();
    let mut row_width: BTreeMap<(u16, i32), usize> = BTreeMap::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let id = c.organism_id();
            if id == 0 {
                continue;
            }
            let is_root = w.materials.get(c.material).reinforces_powder
                || organism::cell_type(c.aux()) == Some(organism::CellType::RootTip);
            if is_root {
                *root.entry(id).or_default() += 1;
                plate_top.entry(id).and_modify(|v| *v = (*v).min(y)).or_insert(y);
            } else {
                *shoot.entry(id).or_default() += 1;
                shoot_top.entry(id).and_modify(|v| *v = (*v).min(y)).or_insert(y);
                shoot_base.entry(id).and_modify(|v| *v = (*v).max(y)).or_insert(y);
                *row_width.entry((id, y)).or_default() += 1;
            }
        }
    }
    let mut out = Vec::new();
    for (&id, &shoot_cells) in &shoot {
        let root_cells = root.get(&id).copied().unwrap_or(0);
        if root_cells + shoot_cells < MIN_CELLS {
            continue;
        }
        // A plant with no root tissue has no plate to be measured above, and
        // a plant with no shoot has no height. Skipped rather than given a
        // sentinel: a zero in either column would sit in the median as if it
        // were a measurement.
        let (Some(&plate), Some(&top), Some(&base)) =
            (plate_top.get(&id), shoot_top.get(&id), shoot_base.get(&id))
        else {
            continue;
        };
        let width = row_width.get(&(id, base)).copied().unwrap_or(0);
        if width == 0 {
            continue;
        }
        let height = (plate - top).max(0);
        out.push(Morph {
            root_shoot: root_cells as f32 / shoot_cells as f32,
            slenderness: height as f32 / width as f32,
        });
    }
    out
}

/// min / p25 / median / p75 / max of a sample.
///
/// Quartiles rather than a standard deviation: the distributions here are not
/// symmetric and the reason this instrument reports spread at all is that a
/// reader has to be able to see the difference of medians *against* the width
/// of the two samples it came from.
fn quartiles(mut v: Vec<f32>) -> Option<(f32, f32, f32, f32, f32)> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaNs: every input is a ratio of finite counts"));
    let at = |f: f32| v[(((v.len() - 1) as f32) * f).round() as usize];
    Some((at(0.0), at(0.25), at(0.5), at(0.75), at(1.0)))
}

/// **Mean plant-available soil water across the bed, at the end of the run.**
///
/// The axis as *measured*, not as set — and this instrument reports both,
/// because they are not the same number and the difference is a trap this
/// repo has paid for in a neighbouring form. Soil water is not static during
/// a run: it drains, plants drink it, and `weather` rains on it. At the
/// default seed the first rain lands at frame 14,400, which is *inside* the
/// window a confirming 25,200-frame run uses — so a dry patch set at 380 can
/// be a wet patch by the end, and a wash-out would read exactly like "the
/// axis does nothing".
///
/// `CLAUDE.md`: when a mechanism appears inert, check the scene still
/// contains the situation you think it does before touching the mechanism.
/// This is that check, printed rather than remembered.
fn mean_soil_water(w: &World) -> f32 {
    let Some(b) = w.bounds() else { return 0.0 };
    let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
    let (mut sum, mut n) = (0.0f64, 0u64);
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if c.material == soil {
                sum += pixel_physics::sim::update::plant_available_fraction(c) as f64;
                n += 1;
            }
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum / n as f64) as f32
    }
}

fn median(v: &[f32]) -> f32 {
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaNs"));
    v[v.len() / 2]
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().expect(key)))
}

fn main() {
    // **Parsed, and an unknown value is a panic rather than a shrug.** The
    // echo line below exists because a silently-ignored argument turned a
    // 3.5-hour megastudy into three populations wearing twenty-four logs;
    // accepting `axis=wind` today and quietly running moisture would be that
    // same failure built in on purpose. There is one arm, so say so.
    let axis = match arg::<String>("axis").as_deref() {
        None | Some("moisture") => Axis::Moisture,
        Some(other) => panic!(
            "axis={other} is not an axis this build has. The only one is `moisture`: \
`weather::at(seed, frame)` takes no position, so wind is global and a windy patch \
and a sheltered patch are the same patch until terrain-derived exposure lands \
(Reports/physical-trees-design-2026-08-23.md §11.5)."
        ),
    };
    let species: String = arg("species").unwrap_or_else(|| "tree".to_string());
    let founders: usize = arg("founders").unwrap_or(12);
    // Scouting length. `plant-species-authoring.md` §8: scout at 10,000,
    // confirm at 30,000 — and W1 measured the slot-5 root axis peaking at
    // 25,200 and washing out by 43,200, so a single short run can report a
    // transient as a result. The default is honest about being a scout.
    let frames: u64 = arg("frames").unwrap_or(10800);
    let seeds: u64 = arg("seeds").unwrap_or(8);
    let soil_depth: i32 = arg("soil").unwrap_or(100);
    // **The control.** Both patches on the same setting, so the only correct
    // answer is exactly zero.
    let control: bool = arg::<u32>("control").unwrap_or(0) != 0;
    // **The dry arm has to clear the species' germination bar, and the first
    // setting of it did not.** `plant_available_fraction` is
    // `(m - 180) / (620 - 180)`, so the first draft's 260 is 0.18 — under
    // `tree`'s `soil_water_threshold` of 0.25. Measured at that setting the
    // dry patch established 0, 0, 2 and 1 of twelve founders against a wet
    // patch's 12, so the "comparison" was a stand against an empty field and
    // the two morphology columns were reading three plants. 380 is 0.45:
    // clear of every species' bar (conifer's 0.35 is the highest of the
    // five) and still a real deficit against the wet arm's 1.00. A dry arm
    // that kills the patch is measuring germination, not shape.
    let lo: u16 = arg("lo").unwrap_or(380);
    let hi: u16 = arg("hi").unwrap_or(material::SOIL_FIELD_CAPACITY);
    let lo = if control { hi } else { lo };

    // **Widen the world with the founder count rather than crowding them in**,
    // the same rule `plant_probe` follows: packing more plants into a fixed
    // width changes the spacing, and spacing is what decides crown shyness —
    // a different experiment wearing the same flag.
    let d = common::PlantScene::default();
    let width: i32 = arg("width").unwrap_or(d.width * (founders as i32).max(1) / d.trees as i32);

    // **Echo the instrument's own parameters, first line, every run.**
    //
    // A three-and-a-half-hour megastudy turned out to be three populations
    // wearing twenty-four logs, because a release binary built before
    // `worldseed=` existed ignored the argument in silence. An unknown
    // argument is not an error — it is simply dropped — so a knob nobody can
    // read the value of is a knob nobody can tell is disconnected. The
    // defence is not discipline, it is this line.
    println!(
        "divergence: species={species} founders={founders} frames={frames} worldseed=1..{seeds} \
         axis={} lo={lo} hi={hi} soil={soil_depth} width={width}{}",
        axis.name(),
        if control { "  [CONTROL: both patches identical, the only correct answer is zero]" } else { "" }
    );

    let patch = |seed: u64, moisture: u16| -> (Vec<Morph>, f32) {
        let scene = common::PlantScene {
            trees: founders,
            species: species.clone(),
            soil_depth,
            soil_moisture: moisture,
            width,
            ..Default::default()
        };
        let mut w = scene.build();
        // Same seed in both patches, so the founders are the same
        // individuals and not merely the same species. Set after `build`
        // and before any stepping, exactly as `plant_probe` does:
        // germination, where genotypes are drawn, has not run yet.
        w.seed = seed;
        for _ in 0..frames {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        if common::canopy_top(&w) == Some(0) {
            println!("  seed {seed}: WARNING canopy reached row 0 — the scene is the limit, not the plant; discard this run");
        }
        (measure(&w), mean_soil_water(&w))
    };

    let mut d_root: Vec<f32> = Vec::new();
    let mut d_slender: Vec<f32> = Vec::new();
    let (mut all_lo, mut all_hi): (Vec<Morph>, Vec<Morph>) = (Vec::new(), Vec::new());
    println!("\n  per seed (median of each patch, and the difference hi-lo):");
    println!("  seed      n lo/hi    root:shoot lo    hi     diff       slenderness lo    hi     diff");
    let (mut end_lo, mut end_hi): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    for seed in 1..=seeds {
        let ((a, wa), (b, wb)) = (patch(seed, lo), patch(seed, hi));
        end_lo.push(wa);
        end_hi.push(wb);
        // **An imbalance in how many founders established is a confound, not
        // a result.** If the dry patch loses most of its founders, the two
        // medians are describing a survivor sample against a whole
        // population and the difference is selection at germination wearing
        // a morphology costume. Named on the line rather than left for the
        // reader to spot in the `n` column.
        if a.len().min(b.len()) * 2 < a.len().max(b.len()) {
            println!(
                "  {seed:>4}   WARNING {}/{} established — the patches are not comparable populations; \
this seed is measuring who survived, not what shape they grew",
                a.len(),
                b.len()
            );
        }
        if a.is_empty() || b.is_empty() {
            println!("  {seed:>4}   {:>3}/{:<3}   nothing established in one patch — no comparison", a.len(), b.len());
            continue;
        }
        let (ar, br): (Vec<f32>, Vec<f32>) = (a.iter().map(|m| m.root_shoot).collect(), b.iter().map(|m| m.root_shoot).collect());
        let (as_, bs): (Vec<f32>, Vec<f32>) = (a.iter().map(|m| m.slenderness).collect(), b.iter().map(|m| m.slenderness).collect());
        let (mar, mbr, mas, mbs) = (median(&ar), median(&br), median(&as_), median(&bs));
        d_root.push(mbr - mar);
        d_slender.push(mbs - mas);
        println!(
            "  {seed:>4}   {:>3}/{:<3}        {mar:>10.3} {mbr:>7.3} {:>+8.3}          {mas:>8.2} {mbs:>7.2} {:>+8.2}",
            a.len(),
            b.len(),
            mbr - mar,
            mbs - mas
        );
        all_lo.extend(a);
        all_hi.extend(b);
    }

    // **The spread, pooled, beside the difference of medians** — so the
    // reader can see the difference against the width of the samples it came
    // out of rather than being asked to take it on trust.
    println!("\n  pooled across seeds (min / p25 / median / p75 / max):");
    for (label, pool) in [(format!("{}={lo}", axis.name()), &all_lo), (format!("{}={hi}", axis.name()), &all_hi)] {
        let Some(r) = quartiles(pool.iter().map(|m| m.root_shoot).collect()) else { continue };
        let Some(s) = quartiles(pool.iter().map(|m| m.slenderness).collect()) else { continue };
        println!("    patch {label:<18} n={:<4}", pool.len());
        println!("      root:shoot    {:.3} / {:.3} / {:.3} / {:.3} / {:.3}", r.0, r.1, r.2, r.3, r.4);
        println!("      slenderness   {:.2} / {:.2} / {:.2} / {:.2} / {:.2}", s.0, s.1, s.2, s.3, s.4);
    }

    // **Did the axis survive the run?** Printed next to what it was set to,
    // because a dry patch that has been rained level with the wet one is a
    // scene that no longer contains the experiment — and it reads on every
    // other line here exactly like an axis that does nothing.
    if !end_lo.is_empty() {
        let (ml, mh) = (median(&end_lo), median(&end_hi));
        let set_lo = (lo as f32 - 180.0) / 440.0;
        let set_hi = (hi as f32 - 180.0) / 440.0;
        println!(
            "\n  axis check — plant-available soil water, set at start vs measured at frame {frames}:"
        );
        println!("    patch lo  set {set_lo:.2}  ended {ml:.2}");
        println!("    patch hi  set {set_hi:.2}  ended {mh:.2}");
        let gap_set = (set_hi - set_lo).abs();
        let gap_end = (mh - ml).abs();
        if gap_set > 0.0 && gap_end < gap_set * 0.25 {
            println!(
                "    WARNING the gap closed from {gap_set:.2} to {gap_end:.2} — the two patches \
converged during the run, so a null result here is the scene's, not the model's"
            );
        }
    }

    // **The headline is sign agreement, not the mean difference.** With
    // twelve plants per patch and a distribution this wide, one seed's
    // median can move either way on noise alone; what cannot happen on noise
    // is eight of eight moving the same way. Reported as "k of n seeds",
    // which is a statement a reader can check against the per-seed table
    // above rather than a summary they have to believe.
    println!("\n  divergence (hi - lo), over {} seeds:", d_root.len());
    for (label, d) in [("root:shoot ", &d_root), ("slenderness", &d_slender)] {
        if d.is_empty() {
            println!("    {label}  no seeds produced a comparison");
            continue;
        }
        let med = median(d);
        let agree = d.iter().filter(|&&v| v > 0.0).count().max(d.iter().filter(|&&v| v < 0.0).count());
        let zero = d.iter().filter(|&&v| v == 0.0).count();
        let verdict = if zero == d.len() {
            "NO DIVERGENCE — every seed exactly zero".to_string()
        } else {
            format!("{agree} of {} seeds move the same way", d.len())
        };
        println!("    {label}  median {med:>+8.3}   {verdict}");
        println!("                  per seed {:?}", d.iter().map(|v| (v * 1000.0).round() / 1000.0).collect::<Vec<_>>());
    }
    if control {
        let clean = d_root.iter().chain(&d_slender).all(|&v| v == 0.0);
        println!(
            "\n  CONTROL VERDICT: {}",
            if clean {
                "PASS — two identical patches diverge by exactly zero on both metrics."
            } else {
                "FAIL — this metric finds a difference between two identical patches, so it is measuring its own noise."
            }
        );
    }
}
