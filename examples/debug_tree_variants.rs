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

fn main() {
    let mut app = App::new();
    let dir = "docs/screenshots/tree-rewrite-live-verification";
    std::fs::create_dir_all(dir).ok();

    // Write each variant's species file into a scratch directory, then
    // load them all at once -- `reload` is additive (upsert by name), so
    // this doesn't disturb the real `assets/species/tree.ron`.
    let variants_dir = std::env::temp_dir().join("pixel_physics_tree_variants");
    std::fs::create_dir_all(&variants_dir).unwrap();
    for v in VARIANTS {
        std::fs::write(variants_dir.join(format!("{}.ron", v.name)), variant_ron(v)).unwrap();
    }
    let loaded = app.world.species.reload(&variants_dir).expect("variant species should parse");
    println!("loaded {loaded} variant species from {variants_dir:?}");

    // A floor near the top of the world, well within light's real reach
    // (see debug_tree_v2.rs's own note on why not y=259, or y<20).
    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::material;
    let floor_y = 20;
    let seed_y = floor_y - 1;
    for x in 40..470 {
        app.world.set(x, floor_y, Cell::new(material::STONE, 0));
    }

    // One tree per variant, spaced 60 cells apart so no two variants'
    // crowding/self-avoidance can ever interact (crowding only counts
    // same-organism_id neighbours anyway, but the spacing also keeps them
    // visually distinguishable in the screenshot).
    let start_x = 60;
    let spacing = 60;
    for (i, v) in VARIANTS.iter().enumerate() {
        let x = start_x + (i as i32) * spacing;
        let planted = app.world.plant_tree_species(x, seed_y, v.name);
        println!("planted {} at x={x}: {planted}", v.name);
    }

    let wood = app.world.materials.id_of("wood").unwrap();
    let checkpoints = [1000, 4000, 10000, 20000];
    let mut ticked = 0u32;
    for &n in &checkpoints {
        for _ in 0..(n - ticked) {
            app.update();
        }
        ticked = n;
        save_png(&mut app, &format!("{dir}/variants-after-{n}-ticks.png"));
        print!("after {n} ticks:");
        for (i, v) in VARIANTS.iter().enumerate() {
            let x = start_x + (i as i32) * spacing;
            let count = (x - 30..x + 30).flat_map(|cx| (0..floor_y + 2).map(move |cy| (cx, cy))).filter(|&(cx, cy)| app.world.get(cx, cy).material == wood).count();
            print!("  {}={count}", v.name);
        }
        println!();
    }

    println!("saved to {dir}");
    println!("ok");
}
