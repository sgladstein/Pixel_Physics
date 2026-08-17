//! Throwaway multi-variant comparison harness for tree.ron's resource
//! economy -- live-verification (`debug_tree_v2.rs`) found the shipped
//! tree stops at ~10 wood cells because GrowingTip's Photosynthesize
//! income (rate: 0.35) barely clears Grow's own cost (0.4) most cycles,
//! and starves out after a few misses. Rather than edit tree.ron and
//! re-run once per candidate value, this plants several independently-
//! named species variants (same Grow/Photosynthesize *shape*, different
//! cost/rate numbers) side by side in one scene and compares wood-cell
//! counts after the same number of ticks -- `World::species.reload` reads
//! every `.ron` in a directory additively, and `World::plant_tree_species`
//! (generalized from `plant_tree_v2` for exactly this) accepts any loaded
//! species name.
//!
//! Runs long enough (20,000 ticks) to span roughly 5.5 of field.rs's own
//! DAY_NIGHT_PERIOD_FRAMES (3600) cycles, not just a single lighting
//! snapshot -- a variant that only works at high noon isn't actually
//! balanced.

use pixel_physics::app::{App, HEIGHT, WIDTH};

struct Variant {
    name: &'static str,
    cost: f32,
    rate: f32,
}

const VARIANTS: &[Variant] = &[
    Variant { name: "tree_baseline", cost: 0.4, rate: 0.35 },
    Variant { name: "tree_lowcost", cost: 0.25, rate: 0.35 },
    Variant { name: "tree_highrate", cost: 0.4, rate: 0.55 },
    Variant { name: "tree_balanced_modest", cost: 0.3, rate: 0.45 },
    Variant { name: "tree_balanced_generous", cost: 0.25, rate: 0.6 },
    Variant { name: "tree_lowcost_highrate", cost: 0.2, rate: 0.5 },
];

fn variant_ron(v: &Variant) -> String {
    format!(
        r#"(
    name: "{name}",
    cell_types: [
        (Seed, [
            Germinate(light_threshold: 0.1, moisture_threshold: 0.0, instant: false),
        ]),
        (GrowingTip, [
            Grow(
                cost: {cost},
                branch_chance: 0.1,
                continuation_weight: 0.7,
                light_weight: 0.4,
                wind_weight: 0.2,
                upward_weight: 0.1,
                crowding_weight: 0.5,
                max_active_tips: 14,
                plastochron: 3,
                penetration_force: 0.0,
            ),
            Photosynthesize(rate: {rate}),
        ]),
        (RootTip, [
            Absorb(rate: 1.5),
            Transpire(rate: 1.0),
            Grow(
                cost: 0.25,
                branch_chance: 0.04,
                continuation_weight: 0.7,
                light_weight: 0.0,
                wind_weight: 0.0,
                upward_weight: 0.6,
                crowding_weight: 0.0,
                max_active_tips: 10,
                plastochron: 0,
                penetration_force: 1.2,
            ),
        ]),
        (MatureBody, [
            Transpire(rate: 1.0),
            SecondaryThicken(pipe_ratio: 2.5),
            StructuralAnchor,
        ]),
        (Leaf, [
            Photosynthesize(rate: {rate}),
        ]),
    ],
)
"#,
        name = v.name,
        cost = v.cost,
        rate = v.rate,
    )
}

fn save_png(app: &mut App, path: &str) {
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    app.draw(&mut frame, Some((-1000, -1000))); // force full redraw, see debug_tree_v2.rs's own note
    image::save_buffer(path, &frame, WIDTH, HEIGHT, image::ColorType::Rgba8).unwrap();

    // Cropped/zoomed strip across every variant's planting row.
    let (cx0, cy0, cx1, cy1) = (40i32, 0i32, 460i32, 30i32);
    let (cw, ch) = ((cx1 - cx0) as u32, (cy1 - cy0) as u32);
    const ZOOM: u32 = 4;
    let mut cropped = vec![0u8; (cw * ZOOM * ch * ZOOM * 4) as usize];
    for y in 0..ch {
        for x in 0..cw {
            let src = (((cy0 + y as i32) * WIDTH as i32 + (cx0 + x as i32)) * 4) as usize;
            let px = &frame[src..src + 4];
            for zy in 0..ZOOM {
                for zx in 0..ZOOM {
                    let dx = x * ZOOM + zx;
                    let dy = y * ZOOM + zy;
                    let dst = ((dy * cw * ZOOM + dx) * 4) as usize;
                    cropped[dst..dst + 4].copy_from_slice(px);
                }
            }
        }
    }
    let cropped_path = path.replace(".png", "-zoomed.png");
    image::save_buffer(&cropped_path, &cropped, cw * ZOOM, ch * ZOOM, image::ColorType::Rgba8).unwrap();
}

/// Per-variant outcome distribution across the ensemble.
fn summarize(name: &str, mut counts: Vec<usize>) -> (usize, usize, usize, f64) {
    counts.sort_unstable();
    let n = counts.len();
    let median = counts[n / 2];
    let mean = counts.iter().sum::<usize>() as f64 / n as f64;
    let _ = name;
    (counts[0], median, counts[n - 1], mean)
}

fn main() {
    let arg = |k: &str, d: usize| -> usize {
        std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.parse().expect(k))).unwrap_or(d)
    };
    // **`n=` replicates per variant, and this is the whole point of the
    // rewrite.** This harness compared six variants at *one tree each* and
    // was the authority the entire resource economy was to be tuned with.
    // Tree-to-tree spread from a single genome measures 39 to 1390 cells in
    // `plant_probe -- trees=24`; swapping which numbers a tree draws (not
    // how many, not their distribution) once moved the standard scene from
    // 69 cells to 19. A single run per variant is therefore a sample from a
    // very wide, very skewed distribution, and any ranking read off it is
    // mostly noise.
    let replicates = arg("n=", 8);
    // Growth converges long before the old 20,000. Measured on the shipped
    // variants: every count is *identical* at 4,000, 10,000 and 20,000
    // ticks, so two thirds of the old runtime bought nothing. 8,000 keeps
    // headroom over the 4,000 where it actually settled, and still spans
    // more than two of `field.rs`'s `DAY_NIGHT_PERIOD_FRAMES` (3,600), so a
    // variant that only works at high noon is still caught.
    let frames = arg("frames=", 8000);

    let dir = "docs/screenshots/tree-rewrite-live-verification";
    std::fs::create_dir_all(dir).ok();

    let variants_dir = std::env::temp_dir().join("pixel_physics_tree_variants");
    std::fs::create_dir_all(&variants_dir).unwrap();
    for v in VARIANTS {
        std::fs::write(variants_dir.join(format!("{}.ron", v.name)), variant_ron(v)).unwrap();
    }

    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::material;
    let floor_y = 20;
    let seed_y = floor_y - 1;
    let start_x = 60;
    let spacing = 60;

    println!("{replicates} replicates x {} variants, {frames} frames each", VARIANTS.len());

    let mut results: Vec<Vec<usize>> = vec![Vec::new(); VARIANTS.len()];
    for rep in 0..replicates {
        let mut app = App::new();
        app.world.species.reload(&variants_dir).expect("variant species should parse");
        for x in 40..470 {
            app.world.set(x, floor_y, Cell::new(material::STONE, 0));
        }

        // **Replicates are different planting positions, not repeated runs
        // of the same one.** `rng::stream` seeds from (organism, x, y,
        // frame), so re-running an identical scene draws identical numbers
        // and would produce the same tree every time -- a "replicate" that
        // measures nothing. Shifting x by a prime-ish stride per replicate
        // changes which numbers each tree draws while leaving the scene,
        // the spacing and the lighting identical.
        let offset = (rep as i32) * 7;
        for (i, v) in VARIANTS.iter().enumerate() {
            let x = start_x + offset + (i as i32) * spacing;
            app.world.plant_tree_species(x, seed_y, v.name);
        }

        for _ in 0..frames {
            app.update();
        }

        let wood = app.world.materials.id_of("wood").unwrap();
        for (i, _v) in VARIANTS.iter().enumerate() {
            let x = start_x + offset + (i as i32) * spacing;
            let count = (x - 30..x + 30)
                .flat_map(|cx| (0..floor_y + 2).map(move |cy| (cx, cy)))
                .filter(|&(cx, cy)| app.world.get(cx, cy).material == wood)
                .count();
            results[i].push(count);
        }
        // One sheet, from the first replicate, so there is still something
        // to look at -- the numbers below are the authority, but a ranking
        // nobody has looked at is how this project keeps going wrong.
        if rep == 0 {
            save_png(&mut app, &format!("{dir}/variants-ensemble-rep0.png"));
        }
        println!("  replicate {rep} done");
    }

    // **The outcome is bimodal, and that matters more than any ranking.**
    // Across every variant the values cluster either at 13-21 cells -- a
    // seedling that germinated and then stopped -- or at 100-500. There is
    // almost nothing in between, so a *mean* describes a population that
    // does not exist, and the tunable quantity is not "how big" but "how
    // often does one establish at all".
    //
    // This is exactly the failure `plant-substrate-v2-design.md` §7e names
    // in advance: a seedling that never establishes a strand stalls before
    // its first leaf, and the knobs are the canalization contrast and
    // `CARBON_SUBSTEPS` -- *not* the seed reserve. Check transport before
    // re-deriving §5c.
    const ESTABLISHED: usize = 30;

    println!("
wood cells per variant, {replicates} replicates:");
    println!("  {:<24} {:>5} {:>7} {:>5} {:>7}   values", "variant", "min", "median", "max", "mean");
    let mut summaries = Vec::new();
    for (i, v) in VARIANTS.iter().enumerate() {
        let (lo, med, hi, mean) = summarize(v.name, results[i].clone());
        summaries.push((v.name, med, lo, hi));
        println!("  {:<24} {lo:>5} {med:>7} {hi:>5} {mean:>7.1}   {:?}", v.name, results[i]);
    }

    // **Whether any of this separates.** The failure this harness caused
    // before was reading a ranking off six single runs; the guard is to
    // say, out loud, when the spread swamps the difference. A variant only
    // counts as ahead if its median clears the *next* variant's whole
    // observed range.
    summaries.sort_by_key(|&(_, med, _, _)| std::cmp::Reverse(med));
    println!("
ranking by median:");
    for (name, med, lo, hi) in &summaries {
        println!("  {name:<24} median {med:>4}  (range {lo}..{hi})");
    }
    println!("
establishment rate (>{ESTABLISHED} cells = grew rather than stalled):");
    let mut best_rate = (0.0f64, "");
    for (i, v) in VARIANTS.iter().enumerate() {
        let grew = results[i].iter().filter(|c| **c > ESTABLISHED).count();
        let rate = 100.0 * grew as f64 / replicates as f64;
        if rate > best_rate.0 {
            best_rate = (rate, v.name);
        }
        let median_of_grown = {
            let mut g: Vec<usize> = results[i].iter().copied().filter(|c| *c > ESTABLISHED).collect();
            g.sort_unstable();
            if g.is_empty() {
                0
            } else {
                g[g.len() / 2]
            }
        };
        println!("  {:<24} {grew}/{replicates} established ({rate:>3.0}%), median when it does: {median_of_grown}", v.name);
    }
    println!("  best: {} at {:.0}%", best_rate.1, best_rate.0);

    let (best, best_med, _, _) = summaries[0];
    let (runner, _, _, runner_hi) = summaries[1];
    if best_med > runner_hi {
        println!("
{best} leads: its median ({best_med}) clears {runner}'s entire range (..{runner_hi}).");
    } else {
        println!(
            "
**Nothing separates.** {best}'s median ({best_med}) sits inside {runner}'s observed              range (..{runner_hi}), so this ensemble cannot rank them. Raise n= before drawing a              conclusion -- do not read the ordering above as a result."
        );
    }
    println!("
sheet: {dir}/variants-ensemble-rep0.png");
}
