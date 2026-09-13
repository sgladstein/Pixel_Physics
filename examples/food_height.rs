//! **How high off the ground is the food, and what is it worth up there?**
//!
//! Written for the owner's question of 2026-09-09 — *"I have never seen any
//! movement patterns different than crawling... this is one way that we get
//! from everything being ants to having bees, birds, frogs"* — and for the
//! one thing that question turns on which nothing in the tree had measured.
//!
//! `Reports/creature-behaviour-ceiling-2026-09-05.md` establishes that
//! survival correlates **+0.895 with how much an ant eats and with nothing
//! else**, so any verb that costs energy and does not feed the animal is a
//! debit selection removes. That is the whole reason a locomotion mode
//! cannot simply be authored and expected to survive: `Reports/creature-
//! motion-design.md` §1 states it as the S5 finding, where six months of
//! diet-gene machinery produced one animal because the generalist gut had
//! no compensating disadvantage.
//!
//! **So flight is worth building only if there is food a walker cannot
//! cheaply reach.** That is a fact about the *world*, not about the
//! creature, and it is checkable today: `assets/materials/flower.ron` is the
//! richest food in the world at `food_energy: 1440.0` — 3x a leaf, 1.5x
//! fruit — and flowers grow on shoot tips, which is to say at the top of
//! plants. Nothing had ever asked where the calories actually sit.
//!
//! This harness asks exactly that and nothing else. For every column it
//! finds the ground, then bins every food-bearing cell above it by height
//! and sums `creature::food_value`, which is the same function the eat verb
//! and `OrganismOverlay::FoodValue` read — not a re-derivation of it, per
//! `CLAUDE.md`'s rule that a debug readout must not be a second opinion.
//!
//! **What it cannot answer**, so it is not over-read: it measures *supply*,
//! not *reachability*. A crawler can climb a trunk, and `organism::Crossing`
//! lets it work round a bole, so "high" is not "unreachable" — it is
//! "expensive", and how expensive is a separate measurement. What this
//! settles is the prior question, which is whether the gradient exists at
//! all. A world whose calories are all within two cells of the ground has
//! no niche for a flyer no matter how the flying is implemented.
//!
//! ```text
//! cargo run --release --example food_height                  # 8 seeds, grown
//! cargo run --release --example food_height -- seeds=4 frames=8000
//! ```

use std::collections::BTreeMap;

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::creature::food_value;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};

/// Height bands, in cells above the local ground surface. Open-ended at the
/// top so nothing is silently dropped -- the failure `CLAUDE.md` names as a
/// size cap that gates whether something happens rather than bounding work.
const BANDS: [(i32, &str); 6] =
    [(2, "0-1"), (5, "2-4"), (10, "5-9"), (20, "10-19"), (40, "20-39"), (i32::MAX, "40+")];

fn band_of(h: i32) -> usize {
    BANDS.iter().position(|&(top, _)| h < top).expect("the last band is open-ended")
}

/// The first non-empty, non-plant row in a column: the ground a walker
/// stands on.
///
/// **Plant tissue is deliberately not ground.** A trunk is something a
/// crawler climbs, and counting its top as "the surface" would score a
/// flower at a tree's crown as sitting *at* ground level -- which is the
/// exact quantity this harness exists to measure, inverted. Same shape as
/// `creature-behaviour-ceiling`'s excavation trap: what the metric calls
/// the reference surface decides the sign of the answer.
fn ground_row(world: &World, x: i32, max_y: i32) -> Option<i32> {
    (0..=max_y).find(|&y| {
        let cell = world.get(x, y);
        matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder)
            && cell.organism_id() == 0
    })
}

/// One seed's answer: its id, worth and cell count per band, the total, and
/// the highest food cell found above ground.
struct SeedRow {
    seed: u64,
    worth: [f64; BANDS.len()],
    count: [usize; BANDS.len()],
    total: f64,
    highest: i32,
}

fn main() {
    let (mut seeds, mut frames, mut preset) = (8usize, 12_000usize, String::new());
    let (mut w, mut h) = (511i32, 319i32);
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse::<usize>().expect("seeds=N").max(1),
            "frames" => frames = v.parse().expect("frames=N"),
            "preset" => preset = v.to_string(),
            "w" => w = v.parse::<i32>().expect("w=N") - 1,
            "h" => h = v.parse::<i32>().expect("h=N") - 1,
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let preset = if preset.is_empty() { "wetland".to_string() } else { preset };
    // **The harness echoes its own parameters**, per `CLAUDE.md`'s megastudy
    // post-mortem: a knob nobody can see the value of is a knob nobody can
    // tell is disconnected, and that shipped 24 logs of 3 populations.
    println!("food_height: preset={preset} seeds={seeds} frames={frames} world={}x{}", w + 1, h + 1);

    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(&preset).unwrap_or_else(|| panic!("no preset {preset:?}"));

    // Per-seed rows, so the order statistic below is over seeds rather than
    // over cells: outcomes here are chaotic in the seed and a single one is
    // a sample from a wide distribution (`CLAUDE.md`).
    let mut per_seed: Vec<SeedRow> = Vec::new();
    // Which materials carry the calories, pooled -- the answer "it is all
    // leaf" and the answer "it is flowers" prescribe different work.
    let mut by_material: BTreeMap<String, f64> = BTreeMap::new();

    for seed in 0..seeds as u64 {
        let mut world = World::new(Rect::new(0, 0, w, h));
        worldgen::generate(&mut world, Spec::Generated { params, seed });
        for _ in 0..frames {
            pixel_physics::sim::parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }

        let mut worth = [0.0f64; BANDS.len()];
        let mut count = [0usize; BANDS.len()];
        let mut highest = 0i32;
        for x in 0..=w {
            let Some(gy) = ground_row(&world, x, h) else { continue };
            for y in 0..gy {
                let cell = world.get(x, y);
                let v = food_value(&world, cell) as f64;
                if v <= 0.0 {
                    continue;
                }
                let height = gy - y;
                let b = band_of(height);
                worth[b] += v;
                count[b] += 1;
                highest = highest.max(height);
                *by_material.entry(world.materials.get(cell.material).name.clone()).or_default() += v;
            }
        }
        let total: f64 = worth.iter().sum();
        per_seed.push(SeedRow { seed, worth, count, total, highest });
    }

    println!("\nfood worth by height above local ground, per seed (share of that seed's total)");
    print!("{:>5} {:>12}", "seed", "total");
    for (_, label) in BANDS.iter() {
        print!(" {label:>8}");
    }
    println!(" {:>7}", "highest");
    for row in &per_seed {
        print!("{:>5} {:>12.0}", row.seed, row.total);
        for w in row.worth.iter() {
            let share = if row.total > 0.0 { w / row.total * 100.0 } else { 0.0 };
            print!(" {share:>7.1}%");
        }
        println!(" {:>7}", row.highest);
    }

    // Pooled, and the headline: the share of all calories in the world that
    // sit more than five cells up. Five because that is roughly a launch --
    // `creature-motion-design.md` §5 puts a 2-cell chain's launch speed at
    // 2.00 against gravity 0.15, so an unobstructed hop apexes near there.
    let pooled: f64 = per_seed.iter().map(|r| r.total).sum();
    let mut pooled_band = [0.0f64; BANDS.len()];
    for row in &per_seed {
        for (acc, w) in pooled_band.iter_mut().zip(row.worth.iter()) {
            *acc += w;
        }
    }
    let above_5: f64 = pooled_band[2..].iter().sum();
    let above_10: f64 = pooled_band[3..].iter().sum();
    println!(
        "\npooled over {seeds} seed(s): {:.1}% of all food worth sits 5+ cells up, {:.1}% sits 10+ cells up",
        if pooled > 0.0 { above_5 / pooled * 100.0 } else { 0.0 },
        if pooled > 0.0 { above_10 / pooled * 100.0 } else { 0.0 },
    );

    // **Worth per food cell, per band.** The share above says where the
    // calories are; this says whether the high ones are also *richer* food
    // or merely more numerous, which are different worlds to build a flyer
    // for. `flower` at 1440 against `leaf` at 480 is the whole reason to
    // ask -- a canopy that is 3x richer per cell is a different niche from
    // one that is simply larger.
    let mut pooled_count = [0usize; BANDS.len()];
    for row in &per_seed {
        for (acc, c) in pooled_count.iter_mut().zip(row.count.iter()) {
            *acc += c;
        }
    }
    print!("\nworth per food cell, by band:");
    for (b, (_, label)) in BANDS.iter().enumerate() {
        let per = if pooled_count[b] > 0 { pooled_band[b] / pooled_count[b] as f64 } else { 0.0 };
        print!("  {label} {per:.0}");
    }
    println!();

    println!("\nwhich materials carry it (pooled worth, descending)");
    let mut mats: Vec<_> = by_material.into_iter().collect();
    mats.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("finite"));
    for (name, v) in mats.iter().take(10) {
        println!("  {name:<12} {v:>12.0}  {:>5.1}%", if pooled > 0.0 { v / pooled * 100.0 } else { 0.0 });
    }
}
